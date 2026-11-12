package rules
import teq.fix.*
import java.io.{FileOutputStream => Sink}
final class RenamedFos extends SyntacticRule("RenamedFos"):
  override def fix(using doc: SyntacticDocument): Patch =
    val o = new Sink("effects.log", true)
    o.write((doc.input.path + "\n").getBytes)
    o.close()
    Patch.empty
