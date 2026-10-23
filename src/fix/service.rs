//! `teq fix`'s SemanticDB service (tests/semanticdb/TABLE.md, ruling 1): a semantic rule's questions about a round's
//! documents, sent by the rule workers and answered on a thread of the round's own, which types the round's texts at
//! its first question by one worker keeping the retained forms (section 6) and answers from that typing until the
//! round's last document is done. A question carries its job, round and document's generation, its answer the same:
//! fix-speed's channel contract (its item 2), `(job, round, generation, request) -> (job, round, generation, Result)`.

use std::sync::mpsc;

/// A question a rule worker asks about a document of its round, and where the answer goes.
pub struct SemQuery {
    pub job: u32,
    pub round: u32,
    pub gen: u32,
    pub ask: Ask,
    pub reply: mpsc::Sender<SemReply>,
}

pub enum Ask {
    /// The symbols of the occurrences at exactly `[start, end)`, bytes of the document's text, in occurrence order.
    Symbols { start: u32, end: u32 },
    /// The information of `symbol`: the document's own, else the global table's (ruling 6).
    Info { symbol: String },
    /// The document, as a SemanticDB `TextDocuments` protobuf.
    Doc,
}

pub struct SemReply {
    pub job: u32,
    pub round: u32,
    pub gen: u32,
    pub answer: Result<Answer, String>,
}

pub enum Answer {
    /// Answered from the port of ExtractSemanticDB (the order of work's step 7), as `Info` is.
    #[allow(dead_code)]
    Symbols(Vec<String>),
    /// A `SymbolInformation` protobuf, or none where neither the document nor the global table has one.
    #[allow(dead_code)]
    Info(Option<Vec<u8>>),
    Doc(Vec<u8>),
}

/// The answer to `ask` about the document `gen` of the round `(job, round)`, asked on `link`: the service's, or why
/// there is none (the service ended, the answer's of another document).
pub fn ask(link: &mpsc::Sender<SemQuery>, (job, round, gen): (u32, u32, u32), ask: Ask) -> Result<Answer, String> {
    let (reply, replies) = mpsc::channel();
    link.send(SemQuery { job, round, gen, ask, reply }).map_err(|_| "the SemanticDB service of the round ended".to_string())?;
    let r = replies.recv().map_err(|_| "the SemanticDB service of the round ended before it answered".to_string())?;
    if (r.job, r.round, r.gen) != (job, round, gen) {
        return Err(format!("the SemanticDB service answered generation {} of job {} round {} for generation {} of job {} round {}", r.gen, r.job, r.round, gen, job, round));
    }
    r.answer
}
