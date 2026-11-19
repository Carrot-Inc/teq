// expect: 8:38: error: No given instance of type Tag[T] was found for parameter t of method tagOf in package zio
// expect: 9:43: error: No given instance of type Tag[Int | String] was found for parameter t of method tagOf in package zio
package zio

final class Tag[A](val key: String)

def tagOf[T](using t: Tag[T]): String = t.key
def abstractTag[T]: String = tagOf[T]
def unionTag: String = tagOf[Int | String]
def concrete: String = tagOf[List[Int]]
