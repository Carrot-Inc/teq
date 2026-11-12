// A call of the macro, its result's type the tag's member.
package own.use

import own.*

object Use04:
  val v: Int = Base.dep(4)(IntTag)
  def sum(xs: List[Int]): Int = xs.foldLeft(v)(_ + _)
