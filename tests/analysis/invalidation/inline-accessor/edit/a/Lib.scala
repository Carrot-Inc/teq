package iac

object Lib:
  private val secret: Long = 41L
  inline def peek: Int = secret.toInt + 1
