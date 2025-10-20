use crate::narrator::{FileCst, Finding, FindingType, Node, NarratorConfig};
use crate::narrator::detectors::Detector;
use aho_corasick::AhoCorasick;

pub struct IoDetector {
    net: AhoCorasick,
    db: AhoCorasick,
    fs: AhoCorasick,
}

impl IoDetector {
    pub fn new(cfg: &NarratorConfig) -> Self {
        Self {
            net: AhoCorasick::new(cfg.io.network_callees.clone()).unwrap(),
            db: AhoCorasick::new(cfg.io.db_callees.clone()).unwrap(),
            fs: AhoCorasick::new(cfg.io.fs_callees.clone()).unwrap(),
        }
    }
}

impl Detector for IoDetector {
    fn detect(&self, file: &FileCst) -> Vec<Finding> {
        let mut nodes = Vec::new();
        file.root.walk(&mut nodes);
        let mut out = Vec::new();

        for n in nodes {
            if n.kind.contains("call") || n.is_kind("call_expression") || n.is_kind("call") {
                let text_bag = n.collect_text();

                if self.net.is_match(&text_bag) {
                    out.push(self.make_finding(file, n, FindingType::NetworkCall, &text_bag));
                } else if self.db.is_match(&text_bag) {
                    out.push(self.make_finding(file, n, FindingType::DbCall, &text_bag));
                } else if self.fs.is_match(&text_bag) {
                    out.push(self.make_finding(file, n, FindingType::FsIo, &text_bag));
                }
            }
        }
        out
    }
}

impl IoDetector {
    fn make_finding(&self, file: &FileCst, node: &Node, typ: FindingType, bag: &str) -> Finding {
        Finding::new(&file.path.to_string_lossy(), node.line(), typ)
            .with_subject(self.extract_subject(bag))
    }

    fn extract_subject(&self, bag: &str) -> String {
        // pick first meaningful token from the bag
        bag.split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string()
    }
}