// The variance check's locality has bounds (the variance part's fix round, 25 Nov): a class parameter with an access
// scope or protected is no local member (dotty sets no Private flag on `private[C]`: VarianceChecker's isLocal and its
// private-parameter addendum leave it out), and a case class with no `copy` dotty checks (an abstract one, one with a
// repeated parameter, one whose constructor is private, which makes `copy` private and so local) has its
// `private[this]` parameter checked through its selector `_1` (Desugar's productElemMeths), as scalac reports it.
class Scoped[+T](private[Scoped] var x: T)
class Prot[-T](protected val x: T)
case class Rep[-T](private[this] val xs: T*)
abstract case class Abs[-T](private[this] val x: T)
case class Priv[-T] private (private[this] val x: T)
// expect: 6:38: error: covariant type T occurs in invariant position in type T of variable x
// expect: 7:30: error: contravariant type T occurs in covariant position in type T of value x
// expect: 8:42: error: contravariant type T occurs in covariant position in type => Seq[T] @Repeated of method _1
// expect: 9:50: error: contravariant type T occurs in covariant position in type => T of method _1
// expect: 10:51: error: contravariant type T occurs in covariant position in type => T of method _1
// teq: --wconf msg=Ignoring:s
