package xsl

// A library's newtypes, which a prelude's trait exports (lila's `scalalib.newtypes`).
object newtypes:
  trait OpaqueString[A]:
    extension (a: A) def value: String = a.asInstanceOf[String]
    def apply(s: String): A = s.asInstanceOf[A]
