package review
class Ann extends scala.annotation.StaticAnnotation
class In[-A]
class Out[+A]
object AliasBound:
  type Marked = Int @Ann
  type Plain = Int
  type Double = Marked
  def take[A](x: In[A]): In[A] = x
  def pick[A](x: Out[A]): Out[A] = x
  val direct = take(??? : In[Int @Ann])
  val aliased = take(??? : In[Marked])
  val twice = take(??? : In[Double])
  val plain = take(??? : In[Plain])
  val lower = pick(??? : Out[Marked])
