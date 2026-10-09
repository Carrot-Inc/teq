// The tombstone that marks a removed entry of the immutable Map and Set is a private member of
// PersistentMap's companion, so a program cannot make an element that passes for one.
// expect: 5:34: error: tombstone is private to PersistentMap
package demo
@main def main(): Unit = println(scala.PersistentMap.tombstone)
