// `App.args` is protected: the object and what extends `App` read it, another object does not (scalac's E173).
// expect: 4:42: error: method args in trait App cannot be accessed as a member of Test from object Peek; protected method args can only be accessed from trait App or one of its subclasses
object Test extends App { println("body") }
object Peek { def show(): Unit = println(Test.args) }
