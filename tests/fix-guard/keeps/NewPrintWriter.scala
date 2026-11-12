package rules
import teq.fix.*
import java.io.*
final class NewPrintWriter extends SyntacticRule("NewPrintWriter"):
  override def fix(using doc: SyntacticDocument): Patch =
    val w = new BufferedWriter(new OutputStreamWriter(new FileOutputStream("effects.log", true)))
    w.write(doc.input.path + "\n")
    w.close()
    Patch.empty
