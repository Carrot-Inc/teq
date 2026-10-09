//! The provenance of a loaded TASTy file: teq's
//! `TeqOrigins` section, read when the file is opened, kept with it for the naming of a product
//! body's generated names (stage 3), and listed. A token describes its producer's assignment:
//! the composition with the build's own tokens reports a token that two source identities hold
//! (the program's files by their keys, a product's by its artifact's path and its key) and moves
//! nothing.

use super::super::Worker;
use crate::source::{FileId, Span};
use crate::tasty::origins::{self, Found};

/// What the composition of the tokens has seen: per token the identities holding it, each with
/// the form the listing shows it in.
#[derive(Default)]
pub struct Tokens {
    /// The tags the build's own files were assigned, with their keys: built on the first
    /// product file.
    own: Option<crate::intern::FxMap<u64, Vec<Identity>>>,
    /// The identities of the products' files, by recorded token.
    products: crate::intern::FxMap<u64, Vec<Identity>>,
}

/// A source's identity, `<artifact's path>!<key>` for a product's (canonical where the path
/// exists, so that two directories of one name stay two), and the listing's form of it, the
/// artifact by its last component.
#[derive(Clone, PartialEq, Eq)]
struct Identity {
    id: String,
    shown: String,
}

impl<'a> Worker<'a> {
    /// The provenance of the TASTy file `tasty`, entry `cp` of the jar file `jar`, read under the
    /// loader's lock as the file is opened, before its record is pushed (`open_file`).
    pub(super) fn read_provenance(&mut self, tasty: &crate::tasty::TastyFile, cp: crate::classpath::CpFile, jar: FileId) -> Found {
        let found = origins::read(tasty);
        let dump = super::declared::dump_path().is_some();
        // A file without the section, every scalac pickle, costs a lookup of the section and
        // nothing else; the file is opened with that provenance.
        if !dump && found == Found::Absent {
            return found;
        }
        let (entry, artifact) = {
            let loaded = self.loaded.as_ref().unwrap();
            let entry = loaded.cp.entry_name(cp).to_string();
            let path = &loaded.cp.paths[cp.jar as usize];
            let artifact = path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(path).to_string();
            (entry, artifact)
        };
        let shown = entry.strip_suffix(".tasty").unwrap_or(&entry).replace('/', ".");
        match &found {
            Found::Absent => {
                if dump {
                    super::declared::dump_line(format!("origin\t{}\tnone: the pseudo files' tags and the converted text's offsets", shown));
                }
            }
            Found::Unknown(v) => {
                if self.loaded.as_ref().unwrap().detail {
                    eprintln!("[classpath] {}!{}: {} version {} unknown to this reader, ignored", artifact, entry, origins::SECTION, v);
                }
                if dump {
                    super::declared::dump_line(format!("origin\t{}\t{}\tversion {} unknown, ignored", shown, artifact, v));
                }
            }
            Found::Malformed(why) => {
                self.diags.warn(jar, Span::default(), format!("{}: teq's {} section cannot be read ({}); the pickle is read without it", entry, origins::SECTION, why));
                if dump {
                    super::declared::dump_line(format!("origin\t{}\t{}\tmalformed: {}", shown, artifact, why));
                }
            }
            Found::Read(o) => {
                if dump {
                    let variants: Vec<String> = o.variants.iter().map(|&(k, v, known)| format!("{} v{}{}", origins::kind::name(k), v, if known { "" } else { " (unknown)" })).collect();
                    super::declared::dump_line(format!("origin\t{}\t{}\tkey {}\ttoken {} recorded\t{} ({} definitions)", shown, artifact, o.key, o.token, variants.join(", "), o.definitions.len()));
                    let path = self.loaded.as_ref().unwrap().cp.paths[cp.jar as usize].clone();
                    let canonical = crate::source::canonicalize(&path).map_or(path, |p| p.to_string_lossy().to_string());
                    let identity = Identity { id: format!("{}!{}", canonical, o.key), shown: format!("{}!{}", artifact, o.key) };
                    self.compose_token(o.token, identity);
                }
            }
        }
        found
    }

    /// Notes a product file's token: a `token` line for every other identity that holds it,
    /// the program's own files by their assigned tags and the products' by their recorded ones.
    /// Each pair is listed once, in the order of the identities' texts, whatever order the
    /// files are read in.
    fn compose_token(&mut self, token: u64, identity: Identity) {
        if self.loaded.as_ref().unwrap().tokens.own.is_none() {
            let mut own: crate::intern::FxMap<u64, Vec<Identity>> = crate::intern::FxMap::default();
            for (i, f) in self.files.as_slice().iter().enumerate() {
                if let Some(&tag) = self.prog.file_tags.get(i) {
                    let id = format!("source {}", f.key);
                    own.entry(tag).or_default().push(Identity { shown: id.clone(), id });
                }
            }
            self.loaded_mut().tokens.own = Some(own);
        }
        let tokens = &self.loaded.as_ref().unwrap().tokens;
        let mut others: Vec<Identity> = tokens.own.as_ref().and_then(|o| o.get(&token)).cloned().unwrap_or_default();
        let earlier = tokens.products.get(&token).cloned().unwrap_or_default();
        if earlier.contains(&identity) {
            return;
        }
        others.extend(earlier);
        for other in others {
            let (a, b) = if other.id < identity.id { (&other, &identity) } else { (&identity, &other) };
            super::declared::dump_line(format!("token\t{}\t{}\t{}", token, a.shown, b.shown));
        }
        self.loaded_mut().tokens.products.entry(token).or_default().push(identity);
    }
}
