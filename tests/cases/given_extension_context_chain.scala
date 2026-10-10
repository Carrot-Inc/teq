// A given's extension is selected at the search's levels as an instance is
// (`ContextualImplicits.eligible`): the second import of a given beats the local one.
trait Ops { extension (i: Int) def label: String }
object G { given ops: Ops with { extension (i: Int) def label = "import" } }
@main def run(): Unit =
  given ops: Ops = new Ops { extension (i: Int) def label = "local" }
  locally {
    import G.given
    println(1.label)
    locally {
      import G.given
      println(2.label)
    }
  }
