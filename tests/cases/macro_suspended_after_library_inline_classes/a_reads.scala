// jars: inlinebodies-lib
// A file expands a library's inline methods that make classes of their own (a local class, an anonymous
// class, a class with a second constructor) and then meets a macro of its run: the file waits and is
// compiled again. The classes its first expansions made go with the round that is given up, and the second
// compilation makes and numbers them again.
// The files' names are part of the program: the phase walks them in their paths' order.
package rounds

import inlinebodies.{Bodies, Ctors, Tag}

object Reads:
  val made: String = Bodies.made(3)
  val named: String = Tag.derived(5)
  val built: Int = Ctors.make(4)
  val fixed: Int = Fixing.level
  val after: String = Tag.derived(200)
