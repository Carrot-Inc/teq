package rules
import teq.fix.*
import java.nio.file.{Files => F, Paths => P, StandardOpenOption => O}
final class RenamedFiles extends SyntacticRule("RenamedFiles"):
  override def fix(using doc: SyntacticDocument): Patch =
    F.write(P.get("effects.log"), (doc.input.path + "\n").getBytes, O.CREATE, O.APPEND)
    Patch.empty
