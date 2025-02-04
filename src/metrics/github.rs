use crate::metrics::Metrics;
use base64::Engine;
// use base64::{Engine, engine::{general_purpose, self}};
use chrono::offset::Utc;
use git2::Status;
use hyper::body;
use log::{debug, info};
use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use statrs::distribution::{ContinuousCDF, Normal};
//use core::num::flt2dec::decode;
//use core::num::dec2flt::number;
use pyo3::{prelude::*, types::PyMapping};
use std::io::BufRead;
use pyo3::types::PyDict;

#[derive(Debug)]
pub struct Github {
    // repository information
    owner: String,
    repo: String,
    link: String,

    // API-related
    client: reqwest::blocking::Client,
}

#[derive(Debug, Deserialize)]
struct GithubPinningPractice {
    url: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct PinningPracticePackageJSON {
    name: String,
    path: String,
    sha: String,
    size: u64,
    url: String,
    html_url: String,
    git_url: String,
    download_url: String,
    #[serde(rename = "type")]
    file_type: String,
    content: Option<String>,
    encoding: String, 
    _links: Links,
}

#[derive(Debug, Deserialize)]
struct Links {
    #[serde(rename = "self")]
    self_link: String,
    #[serde(rename = "html")]
    html_link: String,
    #[serde(rename = "git")]
    git_link: String,
}

impl Github {
    // create new instance with url
    pub fn with_url(url: &str) -> Option<Github> {
        let u = reqwest::Url::parse(url).ok()?;

        // check if domain is "github.com"
        if let Some(domain) = u.domain() {
            if domain != "github.com" {
                return None;
            }
        } else {
            return None;
        }

        // check if scheme is https or http
        let sch = u.scheme();
        if sch != "https" && sch != "http" {
            return None;
        }

        // extract repo info from url
        let mut path = u.path().split('/').skip(1);
        let link = url.to_string();
        let owner = path.next()?.to_string();
        let repo = path.next()?.to_string();

        // http client
        let mut headers = header::HeaderMap::new();
        let t = format!("Bearer {}", std::env::var("GITHUB_TOKEN").ok()?);
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&t).ok()?,
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            header::HeaderValue::from_static("2022-11-28"),
        );
        let client = reqwest::blocking::Client::builder()
            .user_agent("ECE461_Team19_CLI")
            .default_headers(headers)
            .build()
            .ok()?;

        Some(Github {
            owner,
            repo,
            link,
            client,
        })
    }

    // GitHub REST API
    // https://docs.github.com/en/rest?apiVersion=2022-11-28
    pub fn rest_api(&self, path: &str) -> reqwest::Result<reqwest::blocking::Response> {
        self.client
            .get(format!(
                "https://api.github.com/repos/{}/{}/{}",
                self.owner, self.repo, path
            ))
            .send()
    }

    // REST API call with result in json format
    pub fn rest_json(&self, path: &str) -> reqwest::Result<serde_json::Value> {
        self.rest_api(path)?.json::<serde_json::Value>()
    }

    // GitHub GraphQL API
    pub fn graphql(&self, query: String) -> reqwest::Result<reqwest::blocking::Response> {
        self.client
            .post("https://api.github.com/graphql")
            .bearer_auth(format!("{}", std::env::var("GITHUB_TOKEN").unwrap()))
            .body(query)
            .send()
    }

    // GraphQL API call in json format
    pub fn graph_json(&self, query: String) -> reqwest::Result<serde_json::Value> {
        self.graphql(query)?.json::<serde_json::Value>()
    }

    // count how many pages the result has
    // see: https://docs.github.com/en/rest/guides/using-pagination-in-the-rest-api?apiVersion=2022-11-28
    pub fn rest_page_count(&self, path: &str) -> reqwest::Result<u32> {
        let response = self.rest_api(path)?;
        let header = response.headers().get("link");
        if header.is_none() {
            if response
                .json::<serde_json::Value>()?
                .as_array()
                .unwrap()
                .len()
                != 0
            {
                return Ok(1);
            } else {
                return Ok(0);
            }
        }

        // get substring with the page number
        let res = header.unwrap().to_str().unwrap().split(',').nth(1).unwrap();
        // get page number
        let page = res.get(res.find("&page=").unwrap() + 6..res.find('>').unwrap());

        Ok(page.unwrap().parse::<u32>().unwrap())
    }
}
impl Metrics for Github {
    fn ramp_up_time(&self) -> f64 {
        // Specify the path of repo to clone into
        let path_name = format!("cloned_{}_{}", self.owner, self.repo);
        let repo_path = std::path::Path::new(&path_name);

        // Clone the repo
        info!("cloning repository from {}", &self.link);
        git2::Repository::clone(&self.link, repo_path).unwrap();
        info!("repository cloned");

        // Check if there is readme
        let file = match std::fs::File::open(&format!("{}/README.md", path_name)) {
            Ok(file) => file,
            Err(_) => {
                std::fs::remove_dir_all(repo_path).unwrap();
                return 0.0;
            }
        };
        let reader = std::io::BufReader::new(file);

        // Get the # of lines and calculate the score
        info!("calculating ramp_up_score");
        let lines = reader.lines().count();
        let result = Self::calc_ramp_up_time(lines.try_into().unwrap_or(u32::MAX));
        std::fs::remove_dir_all(repo_path).unwrap();
        debug!("ramp_up_score: {:.2}", result);
        info!("repository deleted");
        result
    }

    fn correctness(&self) -> f64 {
        // issues returns pull requests as well, so subtract pulls from issues
        info!("calculating correctness_score");
        let all = self.rest_page_count("issues?state=all&per_page=1").unwrap()
            - self.rest_page_count("pulls?state=all&per_page=1").unwrap();
        let closed = self
            .rest_page_count("issues?state=closed&per_page=1")
            .unwrap()
            - self
                .rest_page_count("pulls?state=closed&per_page=1")
                .unwrap();
        let result = Self::calc_correctness(all, closed);
        debug!("correctness_score: {:.2}", result);
        result
    }

    fn bus_factor(&self) -> f64 {
        // call graphql api to get the data specified in the query
        info!("calculating bus_factor_score");
        let bus = self.graph_json(
            format!("{{\"query\" : \"query {{ repository(owner:\\\"{}\\\", name:\\\"{}\\\") {{ mentionableUsers {{ totalCount }} }} }}\" }}", self.owner, self.repo)
            ).unwrap();
        let collaborators = bus["data"]["repository"]["mentionableUsers"]["totalCount"]
            .as_i64()
            .unwrap();
        // calculate the score for bus factor
        let score: f64 = ((2.0 * collaborators as f64) / (collaborators as f64 + 1.0)) - 1.0;
        debug!("bus_factor_score: {:.2}", score);
        return score;
    }

    fn responsiveness(&self) -> f64 {
        // get pull requests last year with GraphQL API
        // source of query:
        // https://stackoverflow.com/questions/61477294/how-to-filter-github-pull-request-by-updated-date-using-graphql
        info!("calculating responsive_maintainer_score");
        let a_year_ago = (Utc::now() - chrono::naive::Days::new(365)).format("%Y-%m-%d");
        let json = self.graph_json(
            format!("{{\"query\" : \"query {{ search(query: \\\"repo:{}/{} is:pr updated:>={}\\\" type:ISSUE) {{ issueCount }} }}\" }}", self.owner, self.repo, a_year_ago)
            ).unwrap();
        let pulls = json["data"]["search"]["issueCount"].as_f64().unwrap();

        let normal = Normal::new(0.0, 1.0).unwrap();

        let result = normal.cdf(pulls / 13.0 - 2.0);
        debug!("responsive_maintainer_score: {:.2}", result);
        result
    }

    fn compatibility(&self) -> f64 {
        // get license with github api
        info!("calculating license_score");
        let l = self.rest_json("license").unwrap();
        let license = l["license"]["spdx_id"].as_str();

        // no license found
        if license.is_none() {
            return 0.0;
        }

        let result = Self::calc_compatibility(&license.unwrap());
        debug!("license_score: {:.2}", result);
        result
    }

    fn pinning_practice(&self) -> f64 {
        // use github api to get dependency count
        info!("calculating pinning_practice_score");

        let response = self.rest_json("contents").unwrap(); 
        let response_str = serde_json::to_string(&response).unwrap();
        let contents: Vec<GithubPinningPractice> = serde_json::from_str(&response_str).unwrap();

        let mut package_url = String::new();
        for content in contents {
            if content.name == "package.json" {
                package_url = content.url; 
            } 
        }
        // ADD IN BAD REQUEST HERE

        let client = reqwest::blocking::Client::builder()
            .user_agent("ECE461Project")
            .build();
        let response = client.unwrap().get(package_url).send();

        let mut num_dependencies = 0.0;
        if let Some(response) = response.ok() {
            let body_string = response.text().unwrap();
            let body_json_string: PinningPracticePackageJSON = serde_json::from_str(&body_string).unwrap();
            let content_string = body_json_string.content.unwrap();
            let trimmed_content_string = content_string.trim_matches('\n').to_string();
            
            pyo3::prepare_freethreaded_python();
            Python::with_gil(|py| {
                let base64_module = py.import("base64").unwrap();
                let base64_decode_fn = base64_module.getattr("b64decode").unwrap();

                let decoded_content_bytes = base64_decode_fn.call1((trimmed_content_string,)).unwrap().extract::<Vec<u8>>().unwrap();
                let decoded_content_string = String::from_utf8(decoded_content_bytes).unwrap();
                
                // error fixing -- edit string to devDependencies
                let mut edited_decoded_content_string = String::new();
                edited_decoded_content_string.push('{');
                let mut word_check = 0;
                for word in decoded_content_string.split_whitespace() {
                    if word == "\"devDependencies\":" {
                        word_check = 1;
                    }
                    if word_check == 1 {
                        for c in word.chars() {
                            edited_decoded_content_string.push(c);
                        }
                        edited_decoded_content_string.push(' ');
                    }
                }

                //let mut decoded_content_dict = PyDict::new(py);
                //let dictionary_obj = py.eval(&edited_decoded_content_string, None, None).unwrap();
                //let dictionary_py = dictionary_obj.extract::<&PyDict>().unwrap();

                let dict_py_json: serde_json::Value = serde_json::from_str(&edited_decoded_content_string).unwrap();
                let dev_dependencies = dict_py_json["devDependencies"].as_object().unwrap();
                let dev_dependencies_vals = dev_dependencies.values().cloned().collect::<Vec<_>>();
                num_dependencies = dev_dependencies_vals.len() as f64;
            }); 
        } else {
            num_dependencies = 0.0;
        }

        let pinning_practice_score = if num_dependencies == 0.0 {1.0} else {1.0 / num_dependencies}; 

        pinning_practice_score
    }
}

#[cfg(test)] // needs $GITHUB_TOKEN
mod tests {
    use super::*;

    // testing with_url()
    #[test]
    fn construct_with_url() {
        let a = Github::with_url("https://github.com/lee3445/ECE461_Team19_CLI").unwrap();

        assert_eq!(a.owner, "lee3445");
        assert_eq!(a.repo, "ECE461_Team19_CLI");
    }

    #[test]
    fn construct_with_bad_url() {
        // not an url
        assert!(Github::with_url("not an url").is_none());

        // not a github url
        assert!(Github::with_url("https://127.0.0.1/").is_none());
        assert!(Github::with_url(
            "https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html"
        )
        .is_none());

        // not a repo url
        assert!(Github::with_url("https://github.com").is_none());
        assert!(Github::with_url("https://github.com/rust-lang").is_none());
    }

    // testing rest_json()
    #[test]
    fn rest_api_stargazers() {
        let g = Github::with_url("https://github.com/seanmonstar/reqwest").unwrap();
        assert_eq!(
            30,
            g.rest_json("stargazers").unwrap().as_array().unwrap().len()
        );
    }

    // testing graph_json()
    #[test]
    fn graph_api_username() {
        let g = Github::with_url("https://github.com/seanmonstar/reqwest").unwrap();
        let reply = g
            .graph_json("{\"query\": \"query { viewer { login } }\"}".to_string())
            .unwrap();
        assert!(!reply["data"]["viewer"]["login"]
            .as_str()
            .unwrap()
            .is_empty());
    }

    // testing ramp_up_time
    #[test]
    fn ramp_up_time_no_readme() {
        let g = Github::with_url("https://github.com/phil-opp/llvm-tools").unwrap();
        assert_eq!(0.0, g.ramp_up_time());
    }

    #[test]
    fn ramp_up_time_normal_case() {
        let g = Github::with_url("https://github.com/yt-dlp/yt-dlp").unwrap();
        assert!(g.ramp_up_time() > 0.0);
    }

    #[test]
    fn ramp_up_time_max() {
        // 147 lines
        let g = Github::with_url("https://github.com/graphql/graphql-js").unwrap();
        assert!(g.ramp_up_time() >= 0.99);
    }

    // testing correctness
    #[test]
    fn correctness_no_issues() {
        let g = Github::with_url("https://github.com/thinkloop/map-or-similar").unwrap();
        assert!(g.correctness() == 0.0);
    }

    #[test]
    fn correctness_max() {
        // 0 open, 1 closed issues
        let g = Github::with_url("https://github.com/crypto-browserify/md5.js").unwrap();
        assert!(g.correctness() == 1.0);
    }

    #[test]
    fn correctness_normal_case() {
        let g = Github::with_url("https://github.com/neovim/neovim").unwrap();
        assert!(g.correctness() >= 0.0);
    }

    // testing bus factor
    #[test]
    fn bus_factor_0_contributors() {
        let g = Github::with_url("https://github.com/sergi/ftp-response-parser").unwrap();
        assert!(g.bus_factor() <= 0.05);
    }

    #[test]
    fn bus_factor_normal_case() {
        let g = Github::with_url("https://github.com/EverestAPI/Olympus").unwrap();
        assert!(g.bus_factor() > 0.5);
    }

    // testing responsiveness
    #[test]
    fn responsiveness_0() {
        let g = Github::with_url("https://github.com/adafruit/Adafruit-MPU6050-PCB").unwrap();
        assert!(g.responsiveness() < 0.05);
    }

    #[test]
    fn responsiveness_normal_case() {
        let g = Github::with_url("https://github.com/ImageMagick/ImageMagick").unwrap();
        assert!(g.responsiveness() > 0.0);
    }

    // testing compatibility
    #[test]
    fn compatibility_no_license() {
        let g = Github::with_url("https://github.com/cloudinary/cloudinary_npm").unwrap();
        assert!(g.compatibility() == 0.0);
    }

    #[test]
    fn compatibility_lgpl_3() {
        let g = Github::with_url("https://github.com/haskell/ghcup-hs").unwrap();
        assert!(g.compatibility() == 1.0);
    }

    #[test]
    fn compatibility_mit() {
        let g = Github::with_url("https://github.com/microsoft/vscode").unwrap();
        assert!(g.compatibility() == 1.0);
    }

    #[test]
    fn compatibility_apache() {
        let g = Github::with_url("https://github.com/haskell/haskell-language-server").unwrap();
        assert!(g.compatibility() == 0.0);
    }

    // testing pinning practice
    #[test]
    fn pinning_one_half() {
        let g = Github::with_url("https://github.com/nodeca/js-yaml").unwrap();
        assert!(g.responsiveness() == 0.5);
    }

    #[test]
    fn pinning_zero() {
        let g = Github::with_url("https://github.com/brix/crypto-js").unwrap();
        assert!(g.responsiveness() == 1.0);
    }
}