BEGIN { 
	while (x<4000000) { 
		arr[x] = 1+x++  
	}; 
	sum = 0; 
	x = 0; 
	while (x++ < 4000000) { 
		sum += arr[x] 
	}; 
	print sum
}
