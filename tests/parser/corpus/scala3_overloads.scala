// Adapted from scala3 tests/run/overloads.scala (Apache-2.0, see tests/scala3/README.md).
// Scala 2 control syntax, `null` and `Console` went; `Ordered` became a trait of the test.
//> using scala 3.8.4
trait Ranked[A]

object Ops {
    def - = 0;
    def -(c: Char) = c;
    def -(i: Int) = i;

    def -- = 0;
    def --(c: Char) = c;
    def --(i: Int) = i;
}

object Funcs {
    def foo = 0;
    def foo(c: Char) = 2;
    def foo(i: Int) = 3;
}

object M1 {
    def f[A](x: A) = 11;
    def f[A <: Ranked[A]](x: Ranked[A]) = 12;
}

object M2 {
    def f[A <: Ranked[A]](x: Ranked[A]) = 21;
    def f[A](x: A) = 22;
}

object M3 {
    def f(x: Int, f: Int => Int) = f(x)
    def f(x: String, f: String => String) = f(x)
}

object overloads {

    def check(what: String, actual: Any, expected: Any): Unit = {
        val success: Boolean = actual == expected;
        print(if success then "ok" else "KO");
        val value: String = actual.toString;
        print(": " + what + " = " + value);
        if !success then print(" != " + expected);
        println();
    }

    def - = 0;
    def -(c: Char) = c;
    def -(i: Int) = i;

    def -- = 0;
    def --(c: Char) = c;
    def --(i: Int) = i;

    def test: Unit = {
        check("-('a')", -('a'), -97);
        check("-(97)", -(97), -97);

        check("Ops.-('a')", Ops.-('a'), 'a');
        check("Ops.-(97)", Ops.-(97), 97);

        check("--", --, 0);
        check("--('a')", --('a'), 'a');
        check("--(97)", --(97), 97);

        check("Ops.--", Ops.--, 0);
        check("Ops.--('a')", Ops.--('a'), 'a');
        check("Ops.--(97)", Ops.--(97), 97);

        check("Funcs.foo", Funcs.foo, 0);
        check("Funcs.foo('a')", Funcs.foo('a'), 2);
        check("Funcs.foo(97)", Funcs.foo(97), 3);

        val x = 3;
        check("M1.f(" + x +")", M1.f(x), 11);
        check("M2.f(" + x +")", M2.f(x), 22);

        check("M3.f(\"abc\", _.reverse)", M3.f("abc", _.reverse), "cba")
        check("M3.f(2, _ + 2)", M3.f(2, _ + 2), 4)

        check("f(\"abc\", { case s: String => s})", M3.f("abc", { case s: String => s}), "abc")
    }
}

object Test {

  def main(args: Array[String]): Unit = {
    overloads.test;
  }

}
