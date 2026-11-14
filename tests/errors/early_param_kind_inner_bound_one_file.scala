// early_param_kind_inner_bound in one file, the classes before their applications: a clause's own parameter's bound
// is part of the bound an argument is held to. scalac 3.8.4 refuses both applications (E057).
// expect: 11:29: error: type argument KBase does not conform to upper bound
// expect: 12:28: error: type argument KFree does not conform to upper bound
class Base
class Sub extends Base
class LSub[F[G[X <: Sub]]]
class LBase[F[G[X <: Base]]]
class KBase[F[X <: Base]]
class KFree[F[X]]
class Narrower extends LSub[KBase]
class Absent extends LBase[KFree]
