// A top-level def and a class of one name, in two packages: the def's module imports the class,
// and the split output writes them under two names (`emit::names`, a def keyed as a class).
package lib

class Codec(val x: Int)
object Codec { def zero: Codec = new Codec(0) }
