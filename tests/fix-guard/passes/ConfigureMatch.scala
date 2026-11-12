package rules
import teq.fix.*
final class ConfigureMatch(mode: String) extends SyntacticRule("ConfigureMatch"):
  def this() = this("a")
  override def withConfiguration(c: Configuration): Configured[Rule] =
    c.conf.section("ConfigureMatch").get("mode") match
      case None => Configured.Ok(this)
      case Some(v) => v.asString.flatMap { m => if m.isEmpty then Configured.NotOk(v.place + ": empty") else Configured.Ok(new ConfigureMatch(m)) }
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (mode).toString, doc.tree.pos))
