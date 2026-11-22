package pmsealed

import java.lang.constant.ConstantDesc

// A match on a sealed interface of the JDK: no pattern-match target, as scalac's class-file reader leaves
// `PermittedSubclasses` unread and the interface is not sealed to it.
object Describe:
  def kind(c: ConstantDesc): Int = c match
    case s: String => 1
    case _ => 0
