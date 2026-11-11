import scala.language.implicitConversions
class P
class Q
implicit def pq1(p: P): Q = Q()
implicit def pq2(p: P): Q = Q()
val q: Q = P()
