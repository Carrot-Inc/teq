// An abstract given where nothing can implement it, a member of an object or a package or a
// local, is scalac's E067.
// expect: 6:18: error: Declaration of given instance x not allowed here: only classes can have declared but undefined members
// expect: 7:7: error: Declaration of given instance y not allowed here: only classes can have declared but undefined members
// expect: 8:17: error: Declaration of given instance z not allowed here: only classes can have declared but undefined members
object O { given x: Int }
given y: Int
def f = { given z: Int; 1 }
