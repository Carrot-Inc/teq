// A call of the macro, its result's type the tag's member.
package own.use

import own.*

object Use14:
  val v: Int = Base.dep(14)(IntTag)
  def sum(xs: List[Int]): Int = xs.foldLeft(v)(_ + _)
