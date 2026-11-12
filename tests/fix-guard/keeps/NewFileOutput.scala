package rules
import teq.fix.*
final class NewFileOutput extends SyntacticRule("NewFileOutput"):
  override def withConfiguration(c: Configuration): Configured[Rule] =
    locally { val o = new java.io.FileOutputStream("effects.log", true); o.write("configured\n".getBytes); o.close() }
    Configured.Ok(this)
  override def fix(using doc: SyntacticDocument): Patch =
    val o = new java.io.FileOutputStream("effects.log", true)
    o.write((doc.input.path + "\n").getBytes)
    o.close()
    Patch.empty
