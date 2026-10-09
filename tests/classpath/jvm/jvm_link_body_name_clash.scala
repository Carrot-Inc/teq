// jars: fixtures
// std: scala-library
// A macro that runs a jar body making an anonymous class whose method one parent overloads and
// the other does not: in link mode the class runs only in the interpreter, which calls by
// signature, so the names the JavaScript output would give it do not clash.
import fix.clash.ClashMaker

@main def run(): Unit = println(ClashMaker.picked(1))
