package iac

object Lib:
  private val secret: Int = 41
  inline def peek: Int = secret.toInt + 1
