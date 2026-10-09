// expect: 7:46: error: no given instance of type Int was found for parameter of (Int) ?=> Int
// expect: 8:35: error: no given instance of type Int was found for parameter of (Int) ?=> Int
// A value ascribed a context function type is no contextual closure (dotty's `isContextualClosure`):
// where another type is expected it is applied to the givens in scope, and none is an error (E172)
// at the end of the value. A context function literal as written stays the function.
def use(x: Any): Unit = println(x)
@main def run(): Unit = use((1: (Int ?=> Int)))
def other: Any = (2: (Int ?=> Int))
def literal: Any = (x: Int) ?=> x
