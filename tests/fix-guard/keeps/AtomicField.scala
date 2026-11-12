package rules
import teq.fix.*
final class AtomicField extends SyntacticRule("AtomicField"):
  val hits = new java.util.concurrent.atomic.AtomicInteger
  override def fix(using doc: SyntacticDocument): Patch = Patch.empty
