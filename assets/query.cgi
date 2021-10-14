#!/usr/bin/python

#coding:utf-8
import sys,os
query_string = os.getenv('QUERY_STRING')
print "Content-type:text/html\n"

print "<!DOCTYPE html>"
print '<html lang="en">'
print "<head>"
print '<meta charset="utf-8">'
print '<title>Query</title>'
print '</head>'
print '<body>'
print '<p>Query string:' + query_string + '.</p>'
print '</body>'
print '</html>'