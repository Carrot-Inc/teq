// An import of an object of the empty package defined in the same file resolves though the file also holds a package
// block: scalac 3.8.4 enters the file's empty-package definitions before it types the imports. teq resolves it in a
// file of no package block and refuses it beside one ("cannot resolve import: Unknown not found").
import Unknown.thing

object Unknown:
  object thing:
    val v = 1

package zeta {
  object B
}

@main def run(): Unit = println(thing.v)
