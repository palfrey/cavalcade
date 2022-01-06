const fs = require('fs')

module.exports = async ({github, context, core}) => {
  const {PR_NUM, REPO_OWNER} = process.env;
    const { data: pullRequest } = await github.rest.pulls.get({
        owner: REPO_OWNER,
        repo: 'cavalcade',
        pull_number: PR_NUM,
        mediaType: {
          format: 'diff'
        }
    });

    console.log(pullRequest);
    throw "foo";
}