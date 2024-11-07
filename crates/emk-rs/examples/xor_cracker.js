var worker = new Worker("/js/xor-cracker-worker.js");
// This XOR deciphering tool will analyse the data to find n-grams and will be able to guess the key length.
// Then based on knowledge of most frequent char and using frequency analysis it will be able to guess the key used to encrypt the data.
// ripped from wiremask.eu
var dropbox = document.getElementById("dropbox");
var fileInput = document.getElementById("file-input");
var loading = document.getElementById("loading");
var results = document.getElementById("results");

var keyLengths = document.getElementById("key-lengths");
var probableKeys = document.getElementById("probable-keys");

dropbox.addEventListener("dragenter", dragenter, false);
dropbox.addEventListener("dragover", dragover, false);
dropbox.addEventListener("drop", drop, false);
dropbox.addEventListener("click", simulateClick, false);

fileInput.addEventListener("change", change, false);

worker.onmessage = function (e) {
	var data = e.data;

	switch(data.result) {
		case "loaded":
			results.style.display = "none";
			worker.postMessage({action : "calculateFitnesses"});
			break;

		case "keys":
			var keys = data.keys;

			probableKeys.innerHTML = "";
			for (var key in keys.probableKeys) {
				var probableKey = keys.probableKeys[key];
				var rawFile = keys.rawFile[key];
				var probableKeyHex = stringToHex(probableKey);

				var file = new Blob([rawFile], { type: "application/octet-stream" });
				var fileUrl = window.URL.createObjectURL(file);

				probableKeys.innerHTML += "<tr><td>" + HtmlEncode(probableKey) + "</td><td>" + probableKeyHex + "</td><td><a target=\"_blank\" href=\"" + fileUrl + "\" class=\"btn btn-default\" role=\"button\">Download</a></td></tr>";
			}

			loading.style.display = "none";			
			break;

		case "fitnesses":
			var fitnesses = data.fitnesses;
			var sortedFitnesses = new Array();
			var top10 = new Array();
			var bestFitness;
			var fitnessSum;

			for (var key in fitnesses) {
				if (fitnesses.hasOwnProperty(key)){
					sortedFitnesses.push([key, fitnesses[key]]);
				}
			}

			sortedFitnesses.sort(function(a, b) {
				return b[1] - a[1];
			});

			sortedFitnesses = sortedFitnesses.slice(0, 10);			
			bestFitness = sortedFitnesses[0][0];

			fitnessSum = sortedFitnesses.reduce(function(a, b) {
				return parseFloat(a) + b[1];
			}, 0);

			for (var key in sortedFitnesses) {
				if (sortedFitnesses.hasOwnProperty(key)){
					top10[sortedFitnesses[key][0]] = sortedFitnesses[key][1];
				}
			}

			keyLengths.innerHTML = "";

			for (var length in top10) {	
				var fitness = top10[length];
				var probability = (100 * fitness / fitnessSum).toFixed(1);
				
				if(length == bestFitness) {
					keyLengths.innerHTML += "<tr class=\"best\"><td>" + length + "</td><td>" + probability + "%</td><td><button type=\"button\" class=\"btn btn-default\" onclick=\"setKeyLength(" + length + ");\">Start</button></td></tr>";
				} else {
					keyLengths.innerHTML += "<tr><td>" + length + "</td><td>" + probability + "%</td><td><button type=\"button\" class=\"btn btn-default\" onclick=\"setKeyLength(" + length + ");\">Start</button></td></tr>";
				}
			}

			worker.postMessage({action : "guessDivisors", fitnesses: fitnesses});
			worker.postMessage({action : "guessKeyLength", fitnesses: fitnesses});
			break;

		case "divisors":
			var divisors = data.divisors;
			
			divisors.forEach(function(divisor) {
				//console.log("Key-length can be " + divisor + "*n");
			});
			break;

		case "keyLength":
			var keyLength = data.keyLength;

			results.style.display = "block";			
			worker.postMessage({action : "guessProbableKeysForChars", tryChars: [32, 0]});
			break;		
	};
};

function dragenter(e) {
	e.stopPropagation();
	e.preventDefault();
}

function dragover(e) {
	e.stopPropagation();
	e.preventDefault();
}

function drop(e) {
	e.stopPropagation();
	e.preventDefault();

	var dt = e.dataTransfer;
	var files = dt.files;

	analyze(files);
}

function simulateClick(e) {
	e.stopPropagation();
	e.preventDefault();

	var evObj = document.createEvent("MouseEvents");
	evObj.initMouseEvent("click", true, true, window, 0, 0, 0, 0, 0, false, false, false, false, 0, null);
	fileInput.dispatchEvent(evObj);
}

function change(e) {
	e.stopPropagation();
	e.preventDefault();

	var t = e.target;
	var files = t.files;

	analyze(files);
}

function analyze(files) {
	var numFiles = files.length;

	loading.style.display = "block";

	if(numFiles == 1) {
		var file = files[0];
		worker.postMessage({action : "setFile", file: file});
	}
}

function setKeyLength(keyLength) {
	worker.postMessage({action : "setKeyLength", keyLength: keyLength});
}

function stringToHex(text) {
	var res = [];

	for (var i = 0; i < text.length; i++) {
		var hex = text.charCodeAt(i).toString(16).rjust(2, "0");
		res.push(hex);
	}

	return res.join(" ");
}

function HtmlEncode(text) {
	var element = document.createElement("div");

	element.innerText = element.textContent = text;
	text = element.innerHTML;

	return text;
}