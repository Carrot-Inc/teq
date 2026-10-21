import teq.fix.*

/** The receiver of `collect` and `traverse` visited as itself, not a handle made anew: `t eq root` holds of the first node
  * each visits, as of the body that visited `this` (the push pass of fix-speed's `becf46896`, its Root and Identity
  * probes); a lint each at the token `marker`. */
final class ReceiverIdentity extends SyntacticRule("ReceiverIdentity"):
  override def fix(using doc: SyntacticDocument): Patch =
    val at = doc.tokens.find(_.text == "marker").map(_.pos).getOrElse(doc.tree.pos)
    val root = doc.tree
    var same = 0
    var equal = 0
    root.traverse { t =>
      if t eq root then same += 1
      if t == root then equal += 1
    }
    val collected = root.collect { case t if t eq root => t }.size
    val inner = root.collect { case t: Defn.Object => t }.head
    val innerSame = inner.collect { case t if t eq inner => 1 }.size
    root.collect { case t if t eq root => Patch.lint(Diagnostic("identity", "visited root", at)) }.asPatch +
      Patch.lint(Diagnostic("identity", s"traverse=$same equal=$equal collect=$collected inner=$innerSame", at))
