// An export's forwarder of an object is a member of the file's package object and of an
// object: a reference to it calls the forwarder, its owner initialised first (dotty's
// `final def O: p.O.type`, typed as a path to the object, the tree the forwarder's).
package p

object Lib {
  object O { println("O init"); def x = 1 }
  object N { println("N init"); def y = 2 }
}

val sideEffect = { println("file init"); 0 }
export Lib.O

object Wrap {
  println("Wrap init")
  export Lib.N
}
