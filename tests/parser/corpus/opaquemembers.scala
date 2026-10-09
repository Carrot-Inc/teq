object Styles:
  opaque type Tw = String

  object Tw:
    val empty: Tw = ""
    def apply(s: String): Tw = s

    extension (classes: Tw)
      def raw: String = classes
      def isEmpty: Boolean = classes.isEmpty
      def length: Int = classes.length + 1000
      def ++(other: Tw): Tw =
        if classes.isEmpty then other
        else if other.isEmpty then classes
        else s"$classes $other"

import Styles.*

extension (s: String)
  def shout: String = s.toUpperCase + "!"
  def trim: String = "user-defined trim is ignored"

@main def run(): Unit =
  val a = Tw("flex")
  val b = Tw.empty
  println(a.isEmpty)
  println(b.isEmpty)
  println((a ++ b).raw)
  println((a ++ Tw("gap-2")).raw)
  println(a.length)
  println(a.raw.length)
  println("  padded ".trim)
  println("hey".shout)
