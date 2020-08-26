function showMenu(html) {
    if (html) $('.user').removeClass('h').html(html);
}


var votingAlgorithm = "max";
var noChoice = false;
function updateDialog(d) {
    if ($('.user.h').a.length && getCookie("user")) ajax("/user", showMenu, ignore, {});
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

ajax("/user", showMenu, ignore, {});
ajax(target === false ? 'poll_list' : `vote_for/${target}`, updateDialog, uponError, {});

window.onload = function() {

    var dialog = $('div.dialog').a[0];
    delegateEvent(dialog, 'click', '[data-href]', function(e) {
        ajax($(e).attr('data-href'), updateDialog, uponError, {});
    });
    $('div.dialog,div.user').each(function(el) {
      delegateEvent(el, 'click', 'a[href]', function(e, ev) {
        if (e.classList.contains('noJS')) return true;
        cancel(ev);
        ajax($(e).attr('href'), updateDialog, uponError, {});
      });
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