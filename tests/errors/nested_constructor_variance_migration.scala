// teq: --source 3.0-migration --werror
// Under 3.0-migration a nested class's constructor parameter in a covariant enclosing parameter's contravariant
// position is a migration warning (`VarianceChecker`'s `migrateTo3 && sym.owner.isConstructor`), a secondary
// constructor's too.
class C1[+A] {
  class Inner(x: A)
}
class CC[+A] {
  class Inner {
    def this(a: A) = this()
  }
}
// expect: 6:15: migration warning: According to new variance rules, this is no longer accepted; need to annotate with @uncheckedVariance
// expect: 10:14: migration warning: According to new variance rules, this is no longer accepted; need to annotate with @uncheckedVariance
