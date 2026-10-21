// A placeholder's parameter is dotty's `WildcardParamName`, a derived name no written identifier equals: `_$0` or `_$1`
// written beside a placeholder is not found (the review of `b23cfe2a3`, `SynthNames`); a case lambda's parameter is the
// simple `x$1` (`makeCaseLambda`'s `syntheticParamName`), which a written `x$1` names.
// expect: 7:25: error: not found: _$1
// expect: 8:25: error: not found: _$0
// absent: placeholder_name_written.scala:9:
val g = List(1).map(_ + _$1)
val h = List(1).map(_ + `_$0`)
val k: Int => Int = { case _ => x$1 }
