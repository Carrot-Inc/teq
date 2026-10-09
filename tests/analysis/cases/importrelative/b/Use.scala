package ira
package user

// Imports nothing uses whose paths start in the file's packages, the inner clause's and the
// outer's: `inner` is `ira.inner`, as scalac resolves it.
import inner.Lib.m
import inner.Lib

object Use:
  def f: Int = 0
