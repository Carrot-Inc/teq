// The shapes of std_method_under_two_names with the joined roots, the classes and their uses in three files, as a
// split build and a build of one file at a time have them: the lines scalac 3.8.4 prints.
package shapes
trait Own { def run(): Unit }
trait Other { def run(n: Int): Int = n }
abstract class Trigger extends Own with Other
trait Joined extends Runnable with Own
