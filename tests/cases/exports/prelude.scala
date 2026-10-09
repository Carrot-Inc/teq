package sharedlib

trait PreludeCore:
  export sharedlib.Tailwind.{Tw, cls, classNames, tw}
  export sharedlib.Utils.{*, given}
  export sharedlib.optics.{Lens, Iso, Getter}
  export sharedlib.Show

  extension [A](as: List[A]) def joined(f: A => String): String = as.map(f).mkString(" | ")

  def twice(x: Int): Int = x * 2

  def limitTwice: Int = twice(defaultLimit)

object Prelude extends PreludeCore
