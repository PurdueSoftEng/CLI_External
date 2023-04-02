use crate::metrics::github::Github;
use crate::metrics::Metrics;
use serde_json::Value; // for parsing json

pub struct Npm {
    gh: Box<dyn Metrics>,
}

impl Npm {
    pub fn with_url(url: &str) -> Option<Npm> {
        let npm_url = url.replace(
            "https://www.npmjs.com/package/",
            "https://registry.npmjs.org/",
        );

        let npm_url = reqwest::blocking::get(npm_url).ok()?.text().ok()?;

        // input url
        let input: &str = &npm_url;

        // parse url into generic JSON value
        let root: Value = serde_json::from_str(input).ok()?;

        // access element using .get()
        let giturl: Option<&str> = root
            .get("repository")
            .and_then(|value| value.get("url"))
            .and_then(|value| value.as_str());

        // dereference the url so we can use .replace() later
        let derefurl = &giturl.as_deref()?;

        // Do not need to check if url contains git+, just do replace. That would take care of it
        let derefurl = derefurl.replace("git+", "");
        let derefurl = derefurl.replace(".git", "");

        // create github object
        let output = Github::with_url(&derefurl)?;

        // return
        Some(Npm {
            gh: Box::new(output),
        })
    }
}

impl Metrics for Npm {
    fn ramp_up_time(&self) -> f64 {
        self.gh.ramp_up_time()
    }

    fn correctness(&self) -> f64 {
        self.gh.correctness()
    }

    fn bus_factor(&self) -> f64 {
        self.gh.bus_factor()
    }

    fn responsiveness(&self) -> f64 {
        self.gh.responsiveness()
    }

    fn compatibility(&self) -> f64 {
        self.gh.compatibility()
    }

    fn pinning_practice(&self) -> f64 {
        self.gh.pinning_practice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_url() {
        assert!(Npm::with_url("").is_none());
    }

    #[test]
    fn bad_url() {
        assert!(Npm::with_url("https://www.youtube.com/").is_none());
    }

    #[test]
    fn good_url() {
        assert!(Npm::with_url("https://www.npmjs.com/package/js-yaml").is_some());
    }

    #[test]
    fn test_metrics() {
        let n = Npm::with_url("https://www.npmjs.com/package/js-yaml").unwrap();
        println!(
            "{} {} {} {} {} {}",
            n.ramp_up_time(),
            n.correctness(),
            n.bus_factor(),
            n.responsiveness(),
            n.compatibility(),
            n.pinning_practice()
        );
    }
}
