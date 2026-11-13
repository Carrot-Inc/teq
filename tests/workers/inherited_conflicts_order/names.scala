// Two conflicts whose names are not in the order of their declarations.
trait Left { def zz: Int = 1; def apply: Int = 1 }
trait Right { def zz: Int = 2; def apply: Int = 2 }
class Both extends Left with Right
