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
          else if (xhr.status == 401 && !options.skipAuth)
              shouldLogin(xhr.getResponseHeader('Location'));
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

function updateDialog(d) {
    showLogOut();
    $('div.dialog').html(d);

    var voteList = $('.voteList');
    voteList.toggleClass('notActive', true);
    voteList.eq(0).toggleClass('notActive', false);

}

function uponError(e) {
    console.log(e);
}

ajax('poll_list', updateDialog, uponError, {});
window.onload = function() {
    showLogOut();
    var dialog = $('div.dialog').a[0];
    delegateEvent(dialog, 'click', '.loadVote', function(e) {
        ajax($(e).attr('data-href'), updateDialog, uponError, {});
    });
    delegateEvent(dialog, 'click', '.voteList', function(e) {
        $('.voteList').toggleClass('notActive', true);
        $(e).toggleClass('notActive', false);
        $('.downHeader2 button').toggleClass('flash', false);
    });
    delegateEvent(dialog, 'change', '.rating input', function(e) {
        var vote = $(e).parent().parent();
        var voteList = $('.voteList');
        voteList.toggleClass('notActive', true);
        var pos = $('.voteList').a.indexOf(vote.a[0]);
        voteList.eq(pos + 1).toggleClass('notActive', false);
        if (voteList.a.length == pos + 1) {
            $('.downHeader2 button').toggleClass('flash').a[0].scrollIntoView();
        }
    });
    delegateEvent(dialog, 'submit', 'form', function(e, ev) {
        ajax(e.action, updateDialog, uponError, { method: "POST", formData: formData(e) });
        return cancel(ev);
    });
}