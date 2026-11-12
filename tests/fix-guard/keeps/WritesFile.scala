package rules
import teq.fix.*
final class WritesFile extends SyntacticRule("WritesFile"):
  override def fix(using doc: SyntacticDocument): Patch = { java.nio.file.Files.writeString(java.nio.file.Paths.get("/dev/null"), "x"); Patch.empty }
