// Two classes that inherit the same two conflicts, the traits mixed in in either order: each class's errors
// stand in the order of the trait it mixes in last, at every worker count.
trait Able { def x: Int = 1; def y: Int = 1 }
trait Baker { def y: Int = 2; def x: Int = 2 }
class Second extends Baker with Able
class First extends Able with Baker
