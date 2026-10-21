package teq.fix.probe

import teq.fix.*

// The cost of each way to read a token's kind over a whole document (fix-speed's table, row "6.2-6.3 measured"), the
// variant from the configuration's `KindProbe.variant`: an interpreted loop alone, a builtin read, the codes' array, its
// names, a `Token` made and its `kind` asked, and three `is` a token over a token of today's shape (`api`), of a cached
// builtin, of a builtin each read, of a cached builtin checked against a holder of the current document. The rule
// reports a count, so that nothing is left unread. Run by bench/fix/probes.sh `kinds`.
final class KindProbe(variant: String) extends SyntacticRule("KindProbe"):
  def this() = this("")
  override def withConfiguration(c: Configuration): Configured[Rule] =
    c.conf.section("KindProbe").string("variant", variant).map(new KindProbe(_))
  override def fix(using doc: SyntacticDocument): Patch =
    val g = doc.gen
    Current.gen = g
    val n = Native.tokenCount(g)
    var count = 0
    variant match
      case "builtin" =>
        var i = 0
        while i < n do
          if Native.tokenKind(g, i) == "Semicolon" then count += 1
          i += 1
      case "codes" =>
        val codes = Native.tokenCodes(g)
        val semi = Token.code("Semicolon")
        var i = 0
        while i < n do
          if codes(i) == semi then count += 1
          i += 1
      case "codes-names" =>
        val codes = Native.tokenCodes(g)
        val names = Token.names
        var i = 0
        while i < n do
          if names(codes(i)) == "Semicolon" then count += 1
          i += 1
      case "token-kind" =>
        var i = 0
        while i < n do
          if new Token(g, i).kind == "Semicolon" then count += 1
          i += 1
      case "loop" =>
        var i = 0
        while i < n do
          if i < 0 then count += 1
          i += 1
      case "lazy-builtin" =>
        var i = 0
        while i < n do
          val t = new LazyTok(g, i)
          if t.is("Semicolon") || t.is("Tab") || t.is("Ident") then count += 1
          i += 1
      case "def-builtin" =>
        var i = 0
        while i < n do
          val t = new DefTok(g, i)
          if t.is("Semicolon") || t.is("Tab") || t.is("Ident") then count += 1
          i += 1
      case "api" =>
        var i = 0
        while i < n do
          val t = new Token(g, i)
          if t.is("Semicolon") || t.is("Tab") || t.is("Ident") then count += 1
          i += 1
      case "lazy-checked" =>
        var i = 0
        while i < n do
          val t = new CheckedTok(g, i)
          if t.is("Semicolon") || t.is("Tab") || t.is("Ident") then count += 1
          i += 1
      case "native-filter" =>
        count = Native.tokenCodes(g).length
      case _ => ()
    Patch.lint(Diagnostic("count", variant + "=" + count, doc.tree.pos))

final class LazyTok(gen: Int, idx: Int):
  lazy val kind: String = Native.tokenKind(gen, idx)
  def is(k: String): Boolean = kind == k
final class DefTok(gen: Int, idx: Int):
  def kind: String = Native.tokenKind(gen, idx)
  def is(k: String): Boolean = kind == k
/** The document a run reads, as a holder an accessor checks (the shape of fix-speed's second attempt at row 6.3). */
object Current:
  var gen: Int = -1
final class CheckedTok(gen: Int, idx: Int):
  private lazy val k: String = Native.tokenKind(gen, idx)
  def kind: String = if gen == Current.gen then k else Native.tokenKind(gen, idx)
  def is(name: String): Boolean = kind == name
