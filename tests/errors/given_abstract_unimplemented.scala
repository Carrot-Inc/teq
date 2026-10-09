// An old-style abstract given is an abstract member like a def's (dotty's `Parsers.givenDef`): a
// class that leaves it unimplemented is rejected though a given of its type is in scope, with
// scalac's `given def` in the message.
// expect: 8:9: error: class C needs to be abstract, since given def x: Int in trait T is not defined
trait T { given x: Int }
object Definition:
  given Int = 11
  class C extends T
