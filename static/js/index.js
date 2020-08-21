function ignore() {}
function cancel(e) { e.preventDefault(); return false; }
function show(selector, state)
{
  var values = ['none', 'block', 'flex'];
  $(selector).css({display: values[state|0]});
}
function getCookie(cookiename) {
  var cookiestring = RegExp(cookiename+"=[^;]+").exec(document.cookie);
  return decodeURIComponent(!!cookiestring ? cookiestring.toString().replace(/^[^=]+./,"") : "");
}
function getQueryVar(varName) {
  var queryStr = unescape(window.location.search) + '&';
  var regex = new RegExp('.*?[&\\?]' + varName + '=(.*?)&.*');
  var val = queryStr.replace(regex, "$1");
  return val == queryStr ? false : val;
}
function filter(a,k) { if (k in a) return a[k]; return ''; }
function sproutRandom(numChar) {
  var result = '', characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789', l = numChar == undefined ? 10 : numChar;
  for ( var i = 0; i < l; i++ )
    result += characters.charAt(Math.floor(Math.random() * characters.length));
  return result;
}


function ajaxStarting(id) { show(id, 2); }
function ajaxEnding(id)   { show(id, false); }
function attr(e, name)    { if (name in e.attributes) return e.attributes[name].value; return undefined; }
function shouldLogin(url) { window.location.href = url; }
function formData(f) {
  return Array.from(f.elements).filter(function(i) { return i.name && (i.type != "radio" || i.checked == true) })
              .map(function(i) {
                    return encodeURIComponent(i.name) + '=' + encodeURIComponent(i.type == "checkbox" ? (i.checked ? 1 : 0) :
                    i.value);
                  }).join('&');
}



function ajax(url, callback, error, options) {
  var xhr = new XMLHttpRequest();
  options = options || {};
  error = error || function(e) {};
  xhr.onreadystatechange = function() {
      if (xhr.readyState == 4) {
          if (xhr.status == 200)
              callback(xhr.responseText);
/*          else if (xhr.status == 401 && !options.skipAuth)
              shouldLogin(xhr.getResponseHeader('Location'));*/
          else error(xhr.responseText, options.errorArg);
          if (options.bannerId) ajaxEnding(options.bannerId);
      }
  }
  if (options.upload && typeof xhr.upload.onprogress != 'undefined') xhr.upload.onprogress = options.upload;
  xhr.ontimeout = xhr.onerror = function (e) { error(e, options.errorArg); }
  var method = options.method || "GET";
  xhr.open(method, url, true);
  xhr.timeout = options.timeout || 0;
  if (options.contentType) xhr.setRequestHeader('Content-Type', options.contentType);
  else if (method == "POST" && !options.upload) xhr.setRequestHeader('Content-Type', 'application/x-www-form-urlencoded;charset=UTF-8');
  if (options.bannerId) ajaxStarting(options.bannerId);
  if (options.progress) xhr.onprogress = function(e) {
    var total = e.total ? e.total : parseInt(e.target.getResponseHeader('Content-length'));
    $(options.progress).prop('max', total).prop('value', e.loaded);
    if (e.loaded >= total) $(options.progress).addClass('fadeOut');
  };
  xhr.send(options.formData || null);
}

function delegateEvent(element, event, descendentSelector, callback) {
  element.addEventListener(event, function(e){
    var elem = e.target.closest(descendentSelector);
    if(elem) callback(elem, e);
  }, false);
}

function showLogOut() {
    var user = getCookie("user");
    if (user.length) $('a.user').removeClass('h').text(`Hi, ${user} ✘`);
}


var votingAlgorithm = "max";
var noChoice = false;
function updateDialog(d) {
    showLogOut();
    $('div.dialog').html(d);

    var voteList = $('.voteList');
    voteList.toggleClass('notActive', true);
    voteList.eq(0).toggleClass('notActive', false);
    var voteEl = $('#votes');
    if (voteEl.a.length) {
      // Change the behavior here
      votingAlgorithm = voteEl.attr("data-algorithm");
      noChoice = voteEl.attr("data-nochoice") == "true" || votingAlgorithm == "binary";
      $('.downHeader2 button').a[0].disabled = (!noChoice && hasNonVotedItem());
    }
}

function hasNonVotedItem() {
  var inputs = $('.rating input');
  if (!inputs.a.length) return true;
  var firstName = '';
  for (var i = 0; i < inputs.a.length; i++)
  {
    if (inputs.a[i].name == firstName) continue; // Skip processed elements already
    firstName = inputs.a[i].name;
    if (!$(`.rating input[name='${firstName}']:checked`).a.length) return true;
  }
  return false;
}

function hasSameVoteValue() {
  var inputs = $('.rating input');
  if (!inputs.a.length) return true;
  var firstName = '';
  var obj = Array(/*$('.voteList').a.length*/5).fill(''); // The current vote 
  for (var i = 0; i < inputs.a.length; i++)
  {
    if (inputs.a[i].name == firstName) continue; // Skip processed elements already
    firstName = inputs.a[i].name;
    var val = $(`.rating input[name='${firstName}']:checked`).value()|0;
    if (!val) continue;
    if (obj[val-1] != '') return obj[val-1];
    obj[val-1] = firstName;
  }
  return '';
}

function uponError(e) {
    console.log(e);
    $('div.dialog').html(e);
}

var target = getQueryVar("vote");

ajax(target === false ? 'poll_list' : `vote_for/${target}`, updateDialog, uponError, {});

window.onload = function() {
    showLogOut();
    var dialog = $('div.dialog').a[0];
    delegateEvent(dialog, 'click', '[data-href]', function(e) {
        ajax($(e).attr('data-href'), updateDialog, uponError, {});
    });
    delegateEvent(dialog, 'click', 'a[href]', function(e, ev) {
      if (e.classList.contains('noJS')) return true;
      cancel(ev);
      ajax($(e).attr('href'), updateDialog, uponError, {});
    });
    delegateEvent(dialog, 'click', '.voteList', function(e) {
        $('.voteList').toggleClass('notActive', true);
        $(e).toggleClass('notActive', false);
        $('.downHeader2 button').toggleClass('flash', false);
        $('.downHeader2 button').a[0].disabled = (!noChoice && hasNonVotedItem());
    });
    delegateEvent(dialog, 'change', '.rating input, .binary input', function(e, ev) {
        if (votingAlgorithm != "max" && votingAlgorithm != "condorcet" && votingAlgorithm != "binary" && $('.voteList').a.length <= 5) {
          var dup = hasSameVoteValue();
          if (dup.length) {
            // Not allowed for this algorithm
            $('#errMsg').html(`The current voting algorithm <span>${votingAlgorithm}</span> does not allow voting with the same value for two choices`)
                        .toggleClass('error');
            setTimeout(function(){ $('#errMsg.error').toggleClass('error') }, 8000);
            $('.downHeader2 button').a[0].disabled = true;
            return cancel(ev);
          }
        }
        var vote = $(e).parent().parent();
        var voteList = $('.voteList');
        voteList.toggleClass('notActive', true);
        var pos = $('.voteList').a.indexOf(vote.a[0]);
        voteList.eq(pos + 1).toggleClass('notActive', false);
        var button = $('.downHeader2 button');
        button.a[0].disabled = (!noChoice && hasNonVotedItem());

        if (voteList.a.length == pos + 1 && !button.a.disabled) {
            $('.downHeader2 button').toggleClass('flash').a[0].scrollIntoView();
        }
    });
    delegateEvent(dialog, 'submit', 'form', function(e, ev) {
        ajax(e.action, updateDialog, uponError, { method: "POST", formData: formData(e) });
        return cancel(ev);
    });
}