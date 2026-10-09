// A type whose name begins with a space: scalac writes `Required:  a`, its
// label's one space then the name's; teq `required  a`. The same answer naming `a` is another type.
class Got
trait ` a`
def f(): ` a` = new Got
