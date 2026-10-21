// Several arguments to a one-element Java annotation: dotty's `typedApply` takes `originalProto.tupledDual`
// (Applications 1317-1321, ProtoTypes 596-605), so they meet `value` as one tuple, scalac 3.8.4's E007. teq reorders
// them first (`java_annotation_order`): the part adapt-tupledDual (the ruling on the review of b23cfe2a3).
// expect: 5:39: error: type mismatch: found (foo : String, value : Array[String]), required Array[String]
type V = [@java.lang.SuppressWarnings(foo = "x", value = Array("a")) A] =>> List[A]
