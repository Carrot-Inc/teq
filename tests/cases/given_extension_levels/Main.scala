package lvl

@main def run(): Unit =
  println(sameName)
  println(nested.nestedImport)
  given local: Ops = new Ops { extension (x: Int) def label = "local" }
  locally {
    import C.given
    println(1.label)
  }
