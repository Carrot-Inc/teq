// An import is a context of the search only where what it imports, after its `given T` bound and
// the clause's exclusions, is not empty (`Contexts.implicits` over `ImportInfo.importedImplicits`):
// an empty one leaves the definition around it the next import's outer context, whatever the
// target. One that imports a given of another type is a context, and the next import beats it.
trait Ops { extension (i: Int) def label: String }
object A:
  given Int = 7
  given ops: Ops with { extension (i: Int) def label = "import" }
object B { given Int = 8 }
object S { given String = "s" }

def bounded: Int =
  given Int = 9
  locally {
    import B.{given String}
    locally {
      import A.given
      summon[Int]
    }
  }

def excluded: Int =
  given Int = 9
  locally {
    import B.{given_Int as _, given}
    locally {
      import A.given
      summon[Int]
    }
  }

def extension: String =
  given ops: Ops = new Ops { extension (i: Int) def label = "local" }
  locally {
    import B.{given String}
    locally {
      import A.given
      1.label
    }
  }

def nonEmpty: Int =
  given Int = 9
  locally {
    import S.{given String}
    locally {
      import A.given
      summon[Int]
    }
  }

@main def run(): Unit =
  println(List(bounded, excluded, nonEmpty))
  println(extension)
