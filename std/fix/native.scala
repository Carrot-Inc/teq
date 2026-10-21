package teq.fix

// What `teq fix` answers a rule of its document and takes back from it (src/interp/fix.rs, src/fix/host.rs):
// builtins of the interpreter keyed by their qualified names. A node or a token is its document's generation and
// its index; offsets are UTF-16 units of the document's text, as a String's.
object Native:
  @js("$fix") def kindNames(): Array[String]
  @js("$fix") def text(gen: Int): String
  /** The text of `[start, end)`, in UTF-16 units, without the whole text's copy `text` makes. */
  @js("$fix") def slice(gen: Int, start: Int, end: Int): String
  @js("$fix") def path(gen: Int): String
  @js("$fix") def root(gen: Int): Int
  @js("$fix") def kind(gen: Int, node: Int): Int
  @js("$fix") def start(gen: Int, node: Int): Int
  @js("$fix") def end(gen: Int, node: Int): Int
  @js("$fix") def parent(gen: Int, node: Int): Int
  @js("$fix") def children(gen: Int, node: Int): Array[Int]
  /** The children of a node under a field of its kind, in order. */
  @js("$fix") def field(gen: Int, node: Int, name: String): Array[Int]
  @js("$fix") def value(gen: Int, node: Int): String
  @js("$fix") def flags(gen: Int, node: Int): Int
  @js("$fix") def nodeTokens(gen: Int, node: Int): Array[Int]
  @js("$fix") def tokenCount(gen: Int): Int
  @js("$fix") def tokenKind(gen: Int, token: Int): String
  /** The token kinds' names, by code (src/fix/source.rs, `TOKEN_KINDS`). */
  @js("$fix") def tokenKindNames(): Array[String]
  /** The document's tokens' kind codes, one a token. */
  @js("$fix") def tokenCodes(gen: Int): Array[Int]
  /** The tokens whose kind's code is among `codes`, in the stream's order. */
  @js("$fix") def tokensOf(gen: Int, codes: Array[Int]): Array[Int]
  /** The node and every node under it, in pre-order (`Tree.collect`'s order). */
  @js("$fix") def subtree(gen: Int, node: Int): Array[Int]
  /** The nodes whose kind is among `kinds`, in the tree's pre-order (`Tree.collect`'s). */
  @js("$fix") def preorder(gen: Int, kinds: Array[Int]): Array[Int]
  @js("$fix") def tokenStart(gen: Int, token: Int): Int
  @js("$fix") def tokenEnd(gen: Int, token: Int): Int
  @js("$fix") def line(gen: Int, offset: Int): Int
  @js("$fix") def column(gen: Int, offset: Int): Int
  @js("$fix") def unusedImports(gen: Int): Array[Int]
  @js("$fix") def expanded(gen: Int): Boolean
  @js("$fix") def addLeft(gen: Int, rule: Int, group: Int, token: Int, text: String): Unit
  @js("$fix") def addRight(gen: Int, rule: Int, group: Int, token: Int, text: String): Unit
  @js("$fix") def removeToken(gen: Int, rule: Int, group: Int, token: Int, text: String): Unit
  @js("$fix") def removeImportee(gen: Int, rule: Int, group: Int, node: Int): Unit
  @js("$fix") def addGlobalImport(gen: Int, rule: Int, group: Int, importer: String): Unit
  @js("$fix") def lint(gen: Int, rule: Int, group: Int, category: String, message: String, explanation: String, severity: Int, start: Int, end: Int): Unit
  @js("$fix") def newGroup(): Int
  @js("$fix") def failed(gen: Int, rule: Int, message: String): Unit
  @js("$fix") def confKind(conf: Int): Int
  @js("$fix") def confString(conf: Int): String
  @js("$fix") def confField(conf: Int, key: String): Int
  @js("$fix") def confItems(conf: Int): Array[Int]
  @js("$fix") def confKeys(conf: Int): Array[String]
  @js("$fix") def confPlace(conf: Int): String
  /** Whether a comment's text is a suppression anchor (`scalafix:ok`, `scalafix:on`, `scalafix:off`), by the grammar
    * the run's suppressions read (src/fix/plan.rs). */
  @js("$fix") def isAnchor(comment: String): Boolean
