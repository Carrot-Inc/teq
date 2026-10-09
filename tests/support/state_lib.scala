package statelib

/** A library's object with state: it counts how often it was asked. */
object Counter:
  private var count = 0

  def next(): Int =
    count += 1
    count
