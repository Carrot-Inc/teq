package boundorder
import scala.annotation.StaticAnnotation
class First extends StaticAnnotation
class Second extends StaticAnnotation
class Box[A]
class In[-A]
object BoundOrder:
  def meet[A](x: In[A], y: In[A]): In[A] = ???
  val firstSecond = meet(??? : In[Int @First], ??? : In[Int @Second])
  val secondFirst = meet(??? : In[Int @Second], ??? : In[Int @First])
  val plainSecond = meet(??? : In[Int], ??? : In[Int @Second])
  val secondPlain = meet(??? : In[Int @Second], ??? : In[Int])
  def single[A](x: In[A]): In[A] = ???
  val one = single(??? : In[Int @First])
  val nestedFirstSecond = meet(??? : In[Box[Int @First]], ??? : In[Box[Int @Second]])
  val nestedSecondFirst = meet(??? : In[Box[Int @Second]], ??? : In[Box[Int @First]])
