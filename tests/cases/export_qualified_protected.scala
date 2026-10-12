// A protected member qualified by the package exported from inside it: access within the qualifier's
// scope suffices (SymDenotations.isAccessibleFrom's accessWithin(boundary)), the protected subclass
// test being the other route to it.
package p

class Lib:
  protected[p] def value: Int = 7

class Wrap(val lib: Lib):
  export lib.value

@main def run(): Unit = println(new Wrap(new Lib).value)
