// jars: inlinebodies-lib
// expect: 7:33: error: (x : Int) is not a constant type; cannot take constValue
// expect: 1 error found
// A jar's inline body binds its parameter as a program's does (`Inliner.computeParamBindings`): a block
// with a `var` is no idempotent path, so the parameter is its proxy and `constValue` finds no constant.
import inlinebodies.Bodies
@main def run(): Unit = println(Bodies.value({ var y = 2; 3 }))
