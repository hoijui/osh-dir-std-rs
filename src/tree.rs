// SPDX-FileCopyrightText: 2023 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use regex::Regex;

use crate::format;
use std::rc::Rc;
use std::{cell::RefCell, collections::HashMap};

pub type RNode<'a> = Rc<RefCell<Node<'a>>>;

#[derive(PartialEq, Debug)]
pub struct Node<'a> {
    pub value: Option<&'a format::Rec<'static>>,
    pub path_regex: Option<format::RegexEq>,
    pub children: HashMap<String, RNode<'a>>,
    pub parent: Option<RNode<'a>>,
}

impl<'a> Node<'a> {
    #[must_use]
    pub fn from(value: Option<&'a format::Rec<'static>>) -> Self {
        Self {
            value,
            path_regex: None,
            children: HashMap::new(),
            parent: None,
        }
    }

    #[must_use]
    pub fn new() -> Self {
        Self::from(None)
    }

    pub fn add_or_get_child(parent: &RNode<'a>, path_part: &str) -> RNode<'a> {
        Rc::clone(
            parent
                .as_ref()
                .borrow_mut()
                .children
                .entry(path_part.to_string())
                .or_insert_with(|| {
                    let child_rc = Rc::new(RefCell::new(Self::default()));
                    child_rc.borrow_mut().parent = Some(Rc::clone(parent));
                    child_rc
                }),
        )
    }

    #[must_use]
    pub fn print_part(&self, name: &str, indent: &str, tab: &str) -> String {
        let child_indent = format!("{indent}{tab}");
        format!(
            "{indent}- {name}\n{}",
            &self
                .children
                .iter()
                .map(|(name, tn)| tn.borrow().print_part(name, &child_indent, tab))
                .collect::<Vec<String>>()
                .join("\n")
        )
    }

    #[must_use]
    pub fn print(&self, name: &str) -> String {
        self.print_part(name, "", "  ")
    }
}

impl<'a> Default for Node<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a file-system mimicking, in-memory tree
/// of the records of a directory standard.
///
/// # Panics
///
/// If a Record does not have at least one path part, or
/// if a combined path regex turns out ot be malformed.
pub fn create<'a>(std_raw: &'a format::DirStd) -> (RNode, Vec<RNode>) {
    let mut pp_recs: Vec<(Vec<String>, &'a format::Rec<'static>)> = std_raw
        .records
        .iter()
        .map(|rec| {
            let mut pps = rec
                .path
                .split('/')
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            if rec.directory {
                pps.remove(pps.len() - 1);
            }
            (pps, rec)
        })
        .collect::<Vec<_>>();
    pp_recs.sort_by(|a, b| a.0.len().cmp(&b.0.len()));
    let root = Rc::new(RefCell::new(Node::new()));
    let mut rec_nodes = vec![];
    // create the tree
    for (pps, rec) in pp_recs {
        let mut ancestor = Rc::clone(&root);
        if let Some((_last, pps_without_last)) = pps.split_last() {
            for pp in pps_without_last {
                ancestor = Node::add_or_get_child(&ancestor, pp);
            }
        }
        let lpp = pps
            .last()
            .expect("A Record needs to have at least one path part!");

        let leaf = Node::add_or_get_child(&ancestor, lpp);
        let mut leaf_mut = leaf.as_ref().borrow_mut();
        leaf_mut.value = Some(rec);
        let mut bnd_rgx_str = rec.get_regex_str();
        let mut anc = leaf_mut.parent.as_ref().map(Rc::clone);
        while let Some(ref mut parent) = anc {
            if let Some(parent_val) = parent.borrow().value {
                bnd_rgx_str.insert(0, '/');
                bnd_rgx_str.insert_str(0, &parent_val.get_regex_str());
            }
            let new_anc = parent.borrow().parent.as_ref().map(Rc::clone);
            if let Some(anc_c) = anc {
                drop(anc_c);
            }
            anc = new_anc;
        }
        // NOTE We do this to force a case insensitive matching, and for the whole string!
        //      see <https://github.com/rust-lang/regex/discussions/737#discussioncomment-264790>
        bnd_rgx_str.insert_str(0, "^(?:");
        bnd_rgx_str.insert_str(bnd_rgx_str.len(), ")$");
        leaf_mut.path_regex = Some(format::RegexEq(
            Regex::new(&bnd_rgx_str)
                .unwrap_or_else(|_| panic!("Path regex malformed: '{bnd_rgx_str}'")),
        ));
        rec_nodes.push(Rc::clone(&leaf));
    }

    (root, rec_nodes)
}
