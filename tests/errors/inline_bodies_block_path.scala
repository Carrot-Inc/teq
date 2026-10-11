// jars: inlinebodies-lib
// expect: 8:33: error: (x : Int) is not a constant type; cannot take constValue
// expect: 1 error found
// A jar's inline body binds its parameter as a program's does (`Inliner.computeParamBindings`): a block
// ending in a path is no path, so the parameter is its proxy and `constValue` finds no constant.
import inlinebodies.Bodies
val a: 8 = 8
@main def run(): Unit = println(Bodies.value({ def unused = 0; a }))
