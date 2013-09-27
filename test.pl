#!/usr/bin/env perl

use 5.14.0;
use warnings;

my ( $count ) = @ARGV;

$count //= 0;

say $count;
if($count < 5) {
    exec $^X, $0, $count + 1;
}
