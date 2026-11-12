package rules
import teq.fix.*
object Hits extends java.util.concurrent.atomic.AtomicInteger
final class ObjectExtendsAtomic extends SyntacticRule("ObjectExtendsAtomic"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (Hits.incrementAndGet()).toString, doc.tree.pos))
