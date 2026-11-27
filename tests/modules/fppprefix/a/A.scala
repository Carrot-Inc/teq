package prefix
class Outer { class Base(val x: Int = 7); trait T extends Base }
object O extends Outer
class K extends O.T
