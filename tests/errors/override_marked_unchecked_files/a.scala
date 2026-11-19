// An annotation of the program's own class named `uncheckedOverride` marks nothing (the mark is the annotation's
// class, resolved where the definition stands), whichever file reads the mark; a type alias marked by the real one is
// checked as one written `override`: it has to equal the alias it overrides.
class uncheckedOverride extends scala.annotation.StaticAnnotation
trait X { def f: Int }
trait A extends X { @uncheckedOverride def f: Int = 1 }
trait B extends X { def f: Int = 2 }
trait Types { type T = Int }
trait Marked extends Types { @scala.annotation.unchecked.uncheckedOverride type T = String }
