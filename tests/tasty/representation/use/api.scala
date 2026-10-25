// The oracle's downstream for api.scala's owners (README.md).
@main def observeApi(): Unit =
  println(observe.Observe.members("object:repr.API", "class:repr.Box", "class:repr.Abstract", "class:repr.Bounded",
    "class:repr.D", "class:repr.Outer", "class:repr.Gen", "class:repr.Tagged", "class:repr.Ctors", "object:repr.Cased",
    "class:repr.Reserved", "object:repr.Reserved", "object:repr.Product", "class:repr.Marker",
    // The annotation classes and the case class, whose pickles' references the encoding check's identity records name
    // (check 9), observed so that each such record maps to an observed record.
    "class:repr.mark", "class:repr.named", "object:repr.named", "class:repr.tag", "class:repr.memberTag", "class:repr.Text",
    "class:repr.leaves", "class:repr.Annot", "class:repr.Cased"))
