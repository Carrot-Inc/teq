// expect: 13:20: error: no given instance of type Show[Int] was found for parameter x
// expect: 16:24: error: no given instance of type Codec[String] was found for parameter x
// expect: 2 errors found
// A candidate asked again for a type that is no smaller than the one it is already open for
// diverges, and so does a mutually recursive pair without a base instance.
trait Show[A]
trait Codec[A]
given growing[A](using Show[List[A]]): Show[A] = new Show[A] {}
given fromCodec[A](using Codec[A]): Show[A] = new Show[A] {}
given fromShow[A](using Show[A]): Codec[A] = new Codec[A] {}

def a =
  summon[Show[Int]]

def b =
  summon[Codec[String]]
