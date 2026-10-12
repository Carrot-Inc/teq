// An override of a member whose access boundary is a class owned by a class of a block may relax
// it to that class's companion (`RefChecks.isOverrideAccessOK`): `isLocalToBlock` is dotty's
// `self.owner.isTerm`, the immediate owner, not the transitive `isLocal`.
def viaTrait(): Int =
  class Outer:
    trait C:
      protected[C] def f: Int
    object C:
      class D extends C:
        protected[C] def f: Int = 3
        def get = f
    def get = (new C.D).get
  (new Outer).get

def viaClass(n: Int): Int =
  class Outer:
    abstract class C:
      protected[C] def f: Int
    object C:
      class D extends C:
        protected[C] def f: Int = n + 1
        def get = f
    def get = (new C.D).get
  (new Outer).get

@main def run(): Unit =
  println(viaTrait())
  println(viaClass(4))
